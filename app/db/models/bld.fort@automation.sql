-- bld.fort@automation: Future fort (mesh.fort_automation)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.fort_automation','Future fort');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.fort_automation';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.paving',0,'','{"w":5.6,"d":5.6,"h":0.05,"steps":1}'),
  ('scarp','wall','mat.composite_panel',0,'','{"w":5.3,"d":5.3,"h":0.5,"t":0.14,"ring":1,"gap":1.1,"far":0}'),
  ('rampart','wall','mat.white_composite',0,'','{"w":4.8,"d":4.8,"h":0.8,"t":0.7,"ring":1,"gap":1.1}'),
  ('rampart_glow','wall','mat.light_strip',0,'','{"y":0.8,"w":5.02,"d":5.02,"h":0.03,"t":0.04,"ring":1,"gap":1.1,"far":0}'),
  ('parapet','wall','mat.white_composite',1,'','{"y":0.5,"w":5.32,"d":5.32,"h":0.12,"t":0.16,"ring":1,"gap":1.1,"far":0}'),
  ('domes_n','cylinder','mat.white_composite',0,'','{"x":-2.1,"y":0.55,"z":-2.1,"w":0.95,"h":0.45,"segments":8,"repeat":2,"dx":4.2}'),
  ('domes_s','cylinder','mat.white_composite',0,'','{"x":-2.1,"y":0.55,"z":2.1,"w":0.95,"h":0.45,"segments":8,"repeat":2,"dx":4.2}'),
  ('dome_tops_n','dome','mat.white_composite',1,'','{"x":-2.1,"y":1,"z":-2.1,"w":0.95,"h":0.4,"segments":8,"repeat":2,"dx":4.2}'),
  ('dome_tops_s','dome','mat.white_composite',1,'','{"x":-2.1,"y":1,"z":2.1,"w":0.95,"h":0.4,"segments":8,"repeat":2,"dx":4.2}'),
  ('dome_slits_n','cylinder','mat.graphite',0,'','{"x":-2.1,"y":0.82,"z":-2.1,"w":0.97,"h":0.07,"segments":10,"repeat":2,"dx":4.2,"far":0}'),
  ('dome_slits_s','cylinder','mat.graphite',0,'','{"x":-2.1,"y":0.82,"z":2.1,"w":0.97,"h":0.07,"segments":10,"repeat":2,"dx":4.2,"far":0}'),
  ('gate','box','mat.white_composite',0,'','{"z":2.3,"w":1.5,"d":0.7,"h":1.15,"door":0.75,"door_h":0.85,"door_colour":"#2c3a40"}'),
  ('gate_field','box','mat.light_strip',0,'','{"x":0,"y":0.4,"z":2.66,"w":0.75,"d":0.03,"h":0.06}'),
  ('gate_band','box','mat.white_composite',1,'','{"y":0.95,"z":2.3,"w":1.54,"d":0.74,"h":0.14}'),
  ('command','box','mat.composite_panel',0,'','{"y":0.05,"z":-1.05,"w":2.4,"d":1.1,"h":0.8,"door":0.4,"door_h":0.6,"door_colour":"#2c3a40"}'),
  ('command_glass','box','mat.smart_glass',0,'','{"y":0.5,"z":-1.05,"w":2.42,"d":1.12,"h":0.16,"far":0}'),
  ('command_dome','dome','mat.white_composite',0,'','{"y":0.85,"z":-1.05,"w":1,"h":0.5,"segments":10}'),
  ('command_roof','flat_roof','mat.white_composite',0,'','{"y":0.85,"z":-1.05,"w":2.4,"d":1.1,"t":0.04,"overhang":0.02,"far":0}'),
  ('dish_post','cylinder','mat.white_composite',0,'','{"x":0.8,"y":0.89,"z":-1.1,"w":0.06,"h":0.5,"segments":4,"far":0}'),
  ('dish','dome','mat.white_composite',0,'','{"x":0.8,"y":1.39,"z":-1.1,"yaw":20,"w":0.6,"h":0.07,"segments":8,"pitch":45,"far":0}'),
  ('dish_back','dome','mat.white_composite',0,'','{"x":0.8,"y":1.39,"z":-1.1,"yaw":20,"w":0.6,"h":0.17,"segments":8,"pitch":-135,"far":0}'),
  ('mast','cylinder','mat.white_composite',0,'','{"x":-0.8,"y":0.89,"z":-1.1,"w":0.05,"h":1.6,"segments":4,"far":0}'),
  ('mast_light','box','mat.light_strip',0,'','{"x":-0.8,"y":2.49,"z":-1.1,"w":0.07,"d":0.07,"h":0.07,"far":0}'),
  ('hab_w','dome','mat.white_composite',0,'','{"x":-1.3,"y":0.05,"z":0.1,"w":0.95,"h":0.6,"segments":10,"far":0}'),
  ('hab_e','dome','mat.white_composite',0,'','{"x":1.3,"y":0.05,"z":0.1,"w":0.95,"h":0.6,"segments":10,"far":0}'),
  ('pd_base','cylinder','mat.white_composite',0,'','{"x":0,"y":0.05,"z":0.95,"w":0.7,"h":0.3,"segments":8,"far":0}'),
  ('pd_mount','box','mat.white_composite',0,'','{"x":0,"y":0.35,"z":0.95,"w":0.34,"d":0.36,"h":0.26,"taper":0.2}'),
  ('pd_guns','cylinder','mat.graphite',0,'pd_mount','{"x":-0.07,"y":0.18,"w":0.06,"h":0.7,"segments":4,"pitch":35,"repeat":2,"dx":0.14}'),
  ('flag','banner','mat.cloth',1,'','{"x":-0.45,"y":0.05,"z":0.55,"h":3.2,"fw":0.9,"fh":0.6,"pole_colour":"#e8ecec"}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.fort_automation'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.fort'), NULL, (SELECT id FROM def WHERE key='age.automation'), (SELECT id FROM def WHERE key='mesh.fort_automation'), (SELECT id FROM def WHERE key='mat.white_composite'), 100);
