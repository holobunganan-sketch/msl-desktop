import type {Task,WaitingItem,CalendarEvent} from '../types/domain';
export type CalendarProjection=CalendarEvent&{task_id?:number;waiting_id?:number;completed:boolean;time_kind:'appointment'|'deadline'|'followup'};
/** Read-only projection. Dates belong to original items; no duplicate event rows. */
export function projectCalendar(tasks:Task[],waiting:WaitingItem[],start:number,end:number):CalendarProjection[]{
  const events:CalendarProjection[]=[];
  const valid=(v:number|null|undefined):v is number=>typeof v==='number'&&Number.isFinite(v)&&v>0;
  const overlaps=(at:number,until:number|null)=>at<end&&(until??at+1)>start;
  for(const task of tasks){
    const common={work_id:task.work_id,title:task.title,notes:task.notes,location:null,created_at:task.created_at,updated_at:task.updated_at,completed:task.status==='done',task_id:task.id};
    if(valid(task.scheduled_start)&&overlaps(task.scheduled_start,task.scheduled_end)){
      events.push({...common,id:-(task.id*3),start_at:task.scheduled_start,end_at:task.scheduled_end,all_day:false,kind:'work_block',time_kind:'appointment'});
    }
    if(valid(task.due_at)&&task.due_at!==task.scheduled_start&&overlaps(task.due_at,null)){
      events.push({...common,id:-(task.id*3+1),start_at:task.due_at,end_at:null,all_day:true,kind:'deadline',time_kind:'deadline'});
    }
  }
  for(const item of waiting){
    if(valid(item.follow_up_at)&&overlaps(item.follow_up_at,null))events.push({id:-(item.id*3+2),waiting_id:item.id,work_id:item.work_id,title:item.title,notes:item.notes,location:null,created_at:item.created_at,updated_at:item.updated_at,completed:item.status==='resolved',start_at:item.follow_up_at,end_at:null,all_day:true,kind:'other',time_kind:'followup'});
  }
  return events;
}
